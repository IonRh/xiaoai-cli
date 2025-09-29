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
use serde_json::Number;
use sha1::Sha1;

use super::{MinaDevice, MinaDeviceInfo, MinaResponse};

const LOGIN_URL: &str = "https://account.xiaomi.com/pass/";
const LOGIN_UA: &str = "APP/com.xiaomi.mihome APPV/6.0.103 iosPassportSDK/3.9.0 iOS/14.4 miHSTS";
const URL: &str = "https://api2.mina.mi.com/";
const UA: &str = "MiHome/6.0.103 (com.xiaomi.mihome; build:6.0.103.1; iOS 14.4.0) Alamofire/6.0.103 MICO/iOSApp/appStore/6.0.103";

/// 提供通用的 `Mina` 服务请求。
///
/// 要获取与设备相关的服务，请使用 [`Mina::devices`]。
#[derive(Debug)]
pub struct Mina {
    client: Client,
    cookie_store: Arc<CookieStoreMutex>,
}

impl Mina {
    /// 登录以获取 `Mina` 服务。
    pub async fn login(username: &str, password: &str) -> crate::Result<Self> {
        let login_url = Url::parse(LOGIN_URL).unwrap();

        // 预先添加 Cookies
        let mut cookie_store = CookieStore::new(None);
        let mut device_id = random_id(16);
        device_id.make_ascii_uppercase();
        let cookie = Cookie::parse(
            format!("sdkVersion=3.9; deviceId={}", device_id),
            &login_url,
        )?;
        cookie_store.insert(cookie, &login_url)?;
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));

        // 用于登录的 Client
        let client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_store))
            .user_agent(LOGIN_UA)
            .build()?;

        let bytes = client
            .get(format!("{LOGIN_URL}serviceLogin?sid=micoapi&_json=true"))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let login_response: LoginResponse = serde_json::from_slice(&bytes[11..])?;
        trace!("初步登录成功: {login_response:?}");

        let hash = password_hash(&password);
        let form = HashMap::from([
            ("_json", "true"),
            ("qs", &login_response.qs),
            ("sid", &login_response.sid),
            ("_sign", &login_response._sign),
            ("callback", &login_response.callback),
            ("user", &username),
            ("hash", &hash),
        ]);
        let bytes = client
            .post(format!("{LOGIN_URL}serviceLoginAuth2"))
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let auth_response: AuthResponse = serde_json::from_slice(&bytes[11..])?;
        trace!("认证成功: {auth_response:?}");

        // 获取 serviceToken，存于 Cookies
        let client_sign = client_sign(&auth_response);
        let url = format!(
            "{}&clientSign={}",
            &auth_response.location,
            urlencoding::encode(&client_sign)
        );
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
            .user_agent(UA)
            .cookie_provider(Arc::clone(&cookie_store))
            .build()?;

        Ok(Self {
            client,
            cookie_store,
        })
    }

    /// 获取所有可用的 `MinaDevice`。
    ///
    /// `MinaDevice` 内部使用 `Mina` 做请求，因此需要通过 `Arc` 共享 `Mina`。
    pub async fn devices(self: &Arc<Self>) -> crate::Result<Vec<MinaDevice>> {
        let response = self
            .get("admin/v2/device_list?master=0")
            .await?
            .error_for_status()?
            .json::<MinaResponse>()
            .await?
            .error_for_code()?;
        let info: Vec<MinaDeviceInfo> = serde_json::from_value(response.data)?;
        let devices = info
            .into_iter()
            .map(|info| MinaDevice::new(Arc::clone(&self), info))
            .collect();

        Ok(devices)
    }

    /// `Mina` 服务的通用 GET 请求，对 [`reqwest::Client::get`] 的简单封装。
    ///
    /// 会自动在 `relative_url` 前添加合适的域名，因此传入相对 url 即可，也无需前导的斜杠。
    pub async fn get(&self, relative_url: &str) -> reqwest::Result<Response> {
        let request_id = random_id(30);
        let url = format!("{URL}{relative_url}&requestId=app_ios_{request_id}");

        self.client.get(url).send().await
    }

    /// `Mina` 服务的通用 POST 请求，同 [`get`][Mina::get]，但可以带表单数据。
    ///
    /// 大多数情况下，表单数据的键都是已知且固定的，可以表示为字符串字面量。
    /// 而值可能更加动态，因此选择 `HashMap<&str, String>` 作为表单数据的类型。
    pub async fn post(
        &self,
        relative_url: &str,
        mut form: HashMap<&str, String>,
    ) -> reqwest::Result<Response> {
        let request_id = random_id(30);
        form.insert("requestId", format!("app_ios_{request_id}"));
        let url = format!("{URL}{relative_url}");

        self.client.post(url).form(&form).send().await
    }

    /// 保存登录状态到 `writer`。
    ///
    /// 状态被保存为明文的 json，请注意安全性。
    pub fn save<W: Write>(&self, writer: &mut W) -> cookie_store::Result<()> {
        save_incl_expired_and_nonpersistent(&self.cookie_store.lock().unwrap(), writer)
    }

    /// 从 `reader` 读取登录状态。
    ///
    /// **不会**验证登录状态的有效性，如果在请求时出错，请尝试重新 [`login`][Mina::login]。
    pub fn load<R: BufRead>(reader: R) -> cookie_store::Result<Self> {
        let cookie_store = Arc::new(CookieStoreMutex::new(load_all(reader)?));
        let client = Client::builder()
            .user_agent(UA)
            .cookie_provider(Arc::clone(&cookie_store))
            .build()?;

        Ok(Self {
            client,
            cookie_store,
        })
    }
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

fn password_hash(password: &str) -> String {
    let result = Md5::new().chain_update(password).finalize();
    let mut result = base16ct::lower::encode_string(&result);
    result.make_ascii_uppercase();

    result
}

fn client_sign(json: &AuthResponse) -> String {
    let nsec = Sha1::new()
        .chain_update("nonce=")
        .chain_update(json.nonce.to_string())
        .chain_update("&")
        .chain_update(&json.ssecurity)
        .finalize();

    Base64::encode_string(&nsec)
}
