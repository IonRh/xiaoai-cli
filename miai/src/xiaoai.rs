use std::{
    collections::HashMap,
    io::{BufRead, Write},
    sync::Arc,
};

use base64ct::{Base64, Encoding};
use cookie_store::{
    Cookie,
    serde::json::{load_all, save_incl_expired_and_nonpersistent},
};
use log::trace;
use md5::{Digest, Md5};
use rand::{
    distr::{Alphanumeric, SampleString},
    rng,
};
use reqwest::{Client, Response, Url};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde::Deserialize;
use serde_json::{Number, Value};
use sha1::Sha1;

use crate::XiaoaiResponse;

const LOGIN_SERVER: &str = "https://account.xiaomi.com/pass/";
const LOGIN_UA: &str = "APP/com.xiaomi.mihome APPV/6.0.103 iosPassportSDK/3.9.0 iOS/14.4 miHSTS";
const API_SERVER: &str = "https://api2.mina.mi.com/";
const API_UA: &str = "MiHome/6.0.103 (com.xiaomi.mihome; build:6.0.103.1; iOS 14.4.0) Alamofire/6.0.103 MICO/iOSApp/appStore/6.0.103";

/// 提供通用的小爱服务请求。
/// 
/// `Xiaoai` 代表着一个账号的登录状态，但如果需要重用的话，也无需再包一层
/// [`std::rc::Rc`] 或 [`Arc`]，`Xiaoai` 已经在内部使用 [`Arc`] 共享状态。
#[derive(Clone, Debug)]
pub struct Xiaoai {
    client: Client,
    cookie_store: Arc<CookieStoreMutex>,
    api_server: Url,
}

impl Xiaoai {
    /// 登录以调用小爱服务。
    pub async fn login(username: &str, password: &str) -> crate::Result<Self> {
        let login_server = Url::parse(LOGIN_SERVER)?;

        // 预先添加 Cookies
        let mut cookie_store = CookieStore::new(None);
        let mut device_id = random_id(16);
        device_id.make_ascii_uppercase();
        let cookie = Cookie::parse(
            format!("sdkVersion=3.9; deviceId={}", device_id),
            &login_server,
        )?;
        trace!("预先添加 Cookies: {cookie:?}");
        cookie_store.insert(cookie, &login_server)?;
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));

        // 用于登录的 Client
        let client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_store))
            .user_agent(LOGIN_UA)
            .build()?;

        // 初步登录以获取一些认证信息
        let bytes = client
            .get(login_server.join("serviceLogin?sid=micoapi&_json=true")?)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        // 前 11 个字节不知道是什么，后面追加 json 响应体
        let login_response: Value = serde_json::from_slice(&bytes[11..])?;
        trace!("尝试初步登录: {login_response}");

        // 二次认证
        let hash = hash_password(password);
        let login_response: LoginResponse = serde_json::from_value(login_response)?;
        let form = HashMap::from([
            ("_json", "true"),
            ("qs", &login_response.qs),
            ("sid", &login_response.sid),
            ("_sign", &login_response._sign),
            ("callback", &login_response.callback),
            ("user", username),
            ("hash", &hash),
        ]);
        let bytes = client
            .post(login_server.join("serviceLoginAuth2")?)
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let auth_response: serde_json::Value = serde_json::from_slice(&bytes[11..])?;
        trace!("尝试二次认证: {auth_response}");

        // 获取 serviceToken，存于 Cookies
        let auth_response: AuthResponse = serde_json::from_value(auth_response)?;
        let client_sign = client_sign(&auth_response);
        let url = Url::parse_with_params(
            &auth_response.location,
            [("clientSign", &client_sign)]
        )?;
        let response = client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        trace!("尝试获取 serviceToken: {response}");

        // 用于请求的 Client
        let client = Client::builder()
            .user_agent(API_UA)
            .cookie_provider(Arc::clone(&cookie_store))
            .build()?;

        Ok(Self {
            client,
            cookie_store,
            api_server: Url::parse(API_SERVER)?
        })
    }

    /// 列出所有设备的信息。
    pub async fn devices(&self) -> crate::Result<Vec<DeviceInfo>> {
        let devices = self
            .get("admin/v2/device_list?master=0")
            .await?
            .error_for_status()?
            .json::<XiaoaiResponse>()
            .await?
            .error_for_code()?
            .extract_data()?;

        Ok(devices)
    }

    /// 小爱服务的通用 GET 请求。
    ///
    /// API 服务器会和 `uri` 做 [`Url::join`]。
    pub async fn get(&self, uri: &str) -> crate::Result<Response> {
        let request_id = random_id(30);
        let uri = format!("{uri}&requestId=app_ios_{request_id}");
        let url = self.api_server.join(&uri)?;

        Ok(self.client.get(url).send().await?)
    }

    /// 小爱服务的通用 POST 请求，同 [`Xiaoai::get`]，但可以带表单数据。
    ///
    /// 大多数情况下，表单数据的键都是已知且固定的，可以表示为字符串字面量。而值可能更加动态，因此选择 
    /// `HashMap<&str, String>` 作为表单数据的类型。
    pub async fn post(
        &self,
        uri: &str,
        mut form: HashMap<&str, String>,
    ) -> crate::Result<Response> {
        let request_id = random_id(30);
        form.insert("requestId", format!("app_ios_{request_id}"));
        let url = self.api_server.join(uri)?;

        Ok(self.client.post(url).form(&form).send().await?)
    }

    /// 保存登录状态到 `writer`。
    ///
    /// 状态被保存为明文的 json，请注意安全性。
    pub fn save<W: Write>(&self, writer: &mut W) -> cookie_store::Result<()> {
        save_incl_expired_and_nonpersistent(&self.cookie_store.lock().unwrap(), writer)
    }

    /// 从 `reader` 读取登录状态。
    ///
    /// **不会**验证登录状态的有效性，如果在请求时出错，请尝试重新 [`login`][Xiaoai::login]。
    pub fn load<R: BufRead>(reader: R) -> cookie_store::Result<Self> {
        let cookie_store = Arc::new(CookieStoreMutex::new(load_all(reader)?));
        let client = Client::builder()
            .user_agent(API_UA)
            .cookie_provider(Arc::clone(&cookie_store))
            .build()?;

        Ok(Self {
            client,
            cookie_store,
            api_server: Url::parse(API_SERVER).unwrap(),
        })
    }
}

/// 小爱设备信息。
#[derive(Clone, Deserialize, Debug)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: String,
    pub hardware: String,
}

#[derive(Deserialize, Debug)]
struct LoginResponse {
    qs: String,
    sid: String,
    _sign: String,
    callback: String,
}

#[derive(Deserialize, Debug)]
struct AuthResponse {
    location: String,
    nonce: Number,
    ssecurity: String,
}

fn random_id(len: usize) -> String {
    Alphanumeric.sample_string(&mut rng(), len)
}

fn hash_password(password: &str) -> String {
    let result = Md5::new().chain_update(password).finalize();
    let mut result = base16ct::lower::encode_string(&result);
    result.make_ascii_uppercase();

    result
}

fn client_sign(payload: &AuthResponse) -> String {
    let nsec = Sha1::new()
        .chain_update("nonce=")
        .chain_update(payload.nonce.to_string())
        .chain_update("&")
        .chain_update(&payload.ssecurity)
        .finalize();

    Base64::encode_string(&nsec)
}
