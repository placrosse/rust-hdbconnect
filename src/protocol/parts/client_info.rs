use super::hdb_value::{emit_length_and_string, string_length};

use crate::HdbResult;

use std::collections::HashMap;
use std::env;
use std::io;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct ClientInfo(HashMap<ClientInfoKey, String>);

impl Default for ClientInfo {
    fn default() -> ClientInfo {
        let mut ci = ClientInfo(HashMap::<ClientInfoKey, String>::new());

        if let Some(os_str) = env::args_os().next() {
            let p = Path::new(&os_str);
            if let Some(s) = p.file_name() {
                ci.set_application(s.to_string_lossy());
            }
        }
        ci.set_driver(env!("CARGO_PKG_NAME"));
        ci.set_driver_version(env!("CARGO_PKG_VERSION"));
        ci.set_driver_info("rust rocks!");
        ci
    }
}

impl ClientInfo {
    fn set_application<S: AsRef<str>>(&mut self, application: S) {
        self.set(ClientInfoKey::Application, application.as_ref());
    }
    pub fn set_application_version(&mut self, application_version: &str) {
        self.set(ClientInfoKey::ApplicationVersion, application_version);
    }
    pub fn set_application_source(&mut self, application_source: &str) {
        self.set(ClientInfoKey::ApplicationSource, application_source);
    }
    pub fn set_application_user(&mut self, application_user: &str) {
        self.set(ClientInfoKey::ApplicationUser, application_user);
    }
    fn set_driver(&mut self, driver: &str) {
        self.set(ClientInfoKey::Driver, driver);
    }
    fn set_driver_info(&mut self, driver_info: &str) {
        self.set(ClientInfoKey::DriverInfo, driver_info);
    }
    fn set_driver_version(&mut self, driver_version: &str) {
        self.set(ClientInfoKey::DriverVersion, driver_version);
    }

    pub fn emit<T: io::Write>(&self, w: &mut T) -> HdbResult<()> {
        for (key, value) in &self.0 {
            emit_length_and_string(key.get_string(), w)?;
            emit_length_and_string(value, w)?;
        }
        Ok(())
    }

    pub fn size(&self) -> usize {
        let mut len = 0;
        for (key, value) in &self.0 {
            len += string_length(key.get_string()) + string_length(value);
        }
        len
    }
    pub fn count(&self) -> usize {
        self.0.len() * 2
    }

    fn set(&mut self, key: ClientInfoKey, value: &str) {
        let value = value.to_string();
        self.0.insert(key, value);
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ClientInfoKey {
    Application,
    ApplicationVersion,
    ApplicationSource,
    ApplicationUser,
    Driver,
    DriverInfo,
    DriverVersion,
}
impl ClientInfoKey {
    fn get_string(&self) -> &str {
        match &self {
            ClientInfoKey::Application => "APPLICATION",
            ClientInfoKey::ApplicationVersion => "APPLICATIONVERSION",
            ClientInfoKey::ApplicationSource => "APPLICATIONSOURCE",
            ClientInfoKey::ApplicationUser => "APPLICATIONUSER",
            ClientInfoKey::Driver => "DRIVER",
            ClientInfoKey::DriverInfo => "DRIVERINFO",
            ClientInfoKey::DriverVersion => "DRIVERVERSION",
        }
    }
}
