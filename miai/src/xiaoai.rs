use std::{
    collections::HashMap,
    io::{BufRead, Write},
    sync::Arc,
};

use cookie_store::serde::json::{load_all, save_incl_expired_and_nonpersistent};
use reqwest::{Client, Response, Url};
use reqwest_cookie_store::CookieStoreMutex;
use serde::Deserialize;

use crate::{XiaoaiResponse, login::Login, util::random_id};

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
    server: Url,
}

impl Xiaoai {
    /// 登录以调用小爱服务。
    pub async fn login(username: &str, password: &str) -> crate::Result<Self> {
        let login = Login::new(username, password)?;
        let raw_login_response = login.login().await?;

        let login_response = serde_json::from_value(raw_login_response)?;
        let raw_auth_response = login.auth(login_response).await?;

        let auth_response = serde_json::from_value(raw_auth_response)?;
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
        let url = self.server.join(&uri)?;

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
        let url = self.server.join(uri)?;

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
            server: Url::parse(API_SERVER).unwrap(),
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
