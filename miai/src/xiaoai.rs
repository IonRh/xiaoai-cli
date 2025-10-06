use std::{
    collections::HashMap,
    io::{BufRead, Write},
    sync::Arc,
};

use cookie_store::serde::json::{load_all, save_incl_expired_and_nonpersistent};
use reqwest::{Client, Url};
use reqwest_cookie_store::CookieStoreMutex;
use serde::Deserialize;
use serde_json::json;
use tracing::trace;

use crate::{XiaoaiResponse, login::Login, util::random_id};

const API_SERVER: &str = "https://api2.mina.mi.com/";
const API_UA: &str = "MiHome/6.0.103 (com.xiaomi.mihome; build:6.0.103.1; iOS 14.4.0) Alamofire/6.0.103 MICO/iOSApp/appStore/6.0.103";

/// 提供小爱服务请求。
///
/// `Xiaoai` 代表着一个账号的登录状态，但如果需要重用的话，也无需再包一层
/// [`std::rc::Rc`] 或 [`Arc`]，`Xiaoai` 已经在内部使用 [`Arc`] 共享状态。
#[derive(Clone, Debug)]
pub struct Xiaoai {
    client: Client,
    cookie_store: Arc<CookieStoreMutex>,
    server: Url,
}

impl Xiaoai {
    /// 登录以调用小爱服务。
    pub async fn login(username: &str, password: &str) -> crate::Result<Self> {
        let login = Login::new(username, password)?;
        let login_response = login.login().await?;
        let auth_response = login.auth(login_response).await?;
        login.get_token(auth_response).await?;

        Self::from_login(login)
    }

    /// 从 [`Login`][`crate::login::Login`] 构造。
    pub fn from_login(login: Login) -> crate::Result<Self> {
        let cookie_store = login.into_cookie_store();
        let client = Client::builder()
            .user_agent(API_UA)
            .cookie_provider(cookie_store.clone())
            .build()?;

        Ok(Self {
            client,
            cookie_store,
            server: Url::parse(API_SERVER)?,
        })
    }

    /// 列出所有设备的信息。
    pub async fn device_info(&self) -> crate::Result<Vec<DeviceInfo>> {
        self.raw_device_info().await?.extract_data()
    }

    /// 同 [`Xiaoai::device_info`]，但返回原始的响应。
    pub async fn raw_device_info(&self) -> crate::Result<XiaoaiResponse> {
        let response = self.get("admin/v2/device_list?master=0").await?;
        trace!("获取到设备列表: {}", response.data);

        Ok(response)
    }

    /// 小爱服务的通用 GET 请求。
    ///
    /// API 服务器会和 `uri` 做 [`Url::join`]。
    pub async fn get(&self, uri: &str) -> crate::Result<XiaoaiResponse> {
        let request_id = random_request_id();
        let url =
            Url::parse_with_params(self.server.join(uri)?.as_str(), [("requestId", request_id)])?;
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<XiaoaiResponse>()
            .await?
            .error_for_code()?;

        Ok(response)
    }

    /// 小爱服务的通用 POST 请求。
    ///
    /// 同 [`Xiaoai::get`]，但可以带表单数据。
    pub async fn post(
        &self,
        uri: &str,
        mut form: HashMap<&str, &str>,
    ) -> crate::Result<XiaoaiResponse> {
        let request_id = random_request_id();
        form.insert("requestId", &request_id);
        let url = self.server.join(uri)?;
        let response = self
            .client
            .post(url)
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .json::<XiaoaiResponse>()
            .await?
            .error_for_code()?;

        Ok(response)
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
            server: Url::parse(API_SERVER).unwrap(),
        })
    }

    /// 向小爱设备发送 OpenWrt UBUS 调用请求。
    pub async fn ubus_call(
        &self,
        device_id: &str,
        method: &str,
        path: &str,
        message: &str,
    ) -> crate::Result<XiaoaiResponse> {
        let form = HashMap::from([
            ("deviceId", device_id),
            ("method", method),
            ("path", path),
            ("message", message),
        ]);

        self.post("remote/ubus", form).await
    }

    /// 请求小爱设备播报文本。
    pub async fn text_to_speech(
        &self,
        device_id: &str,
        text: &str,
    ) -> crate::Result<XiaoaiResponse> {
        let message = json!({"text": text}).to_string();

        self.ubus_call(device_id, "text_to_speech", "mibrain", &message)
            .await
    }
}

/// 小爱设备信息。
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// 设备 ID。
    ///
    /// 每个与设备相关的请求都会用 ID 指明对象。
    #[serde(rename = "deviceID")]
    pub device_id: String,

    /// 设备名称。
    pub name: String,

    /// 机型。
    pub hardware: String,
}

fn random_request_id() -> String {
    let mut request_id = random_id(30);
    request_id.insert_str(0, "app_ios_");

    request_id
}
