use error::{Error, Result};
use gssapi_sys;
use name::Name;
use oid::OID;
use oid_set::OIDSet;
use std::ptr;

#[cfg(feature = "services4user")]
use std::ffi::CString;

#[derive(Debug)]
pub struct Credentials {
    cred_handle: gssapi_sys::gss_cred_id_t,
    mechs: OIDSet,
    time_rec: u32,
}

impl Credentials {
    pub fn accept<T: Into<Name>>(desired_name: T) -> CredentialsBuilder {
        CredentialsBuilder::new(desired_name)
    }

    pub fn mechs(&self) -> &OIDSet {
        &self.mechs
    }

    pub fn time_rec(&self) -> u32 {
        self.time_rec
    }

    pub unsafe fn get_handle(&self) -> gssapi_sys::gss_cred_id_t {
        self.cred_handle
    }
    
    #[cfg(feature = "services4user")]
    pub fn impersonate<T: Into<Name>>(self, desired_name: T) -> CredentialsBuilder {
        CredentialsBuilder::new(desired_name).impersonator(self)
    }
    
    #[cfg(feature = "services4user")]
    pub fn store_into(&self, cred_store: &Vec<(CString, CString)>) -> Result<bool> {
        let input_usage = 0;
        let desired_mech = ptr::null_mut();
        let overwrite_cred = 1;
        let default_cred = 0;
        let mut elements_stored = try!(OIDSet::empty());
        let mut cred_usage_stored = 0;
        let mut minor_status = 0;
        
        let mut elements: Vec<gssapi_sys::gss_key_value_element_desc> = cred_store.into_iter()
            .map(|&(ref e1, ref e2)| {
                gssapi_sys::gss_key_value_element_struct {
                    key: e1.as_ptr(),
                    value: e2.as_ptr(),
                } as gssapi_sys::gss_key_value_element_desc
            })
            .collect();
        
        let elements_ptr : *mut gssapi_sys::gss_key_value_element_desc = elements.as_mut_ptr();
        
        let mut gss_cred_store = gssapi_sys::gss_key_value_set_struct {
            count: cred_store.len() as u32,
            elements: elements_ptr,
        };

        let major_status = unsafe {
            // Example usage: https://github.com/krb5/krb5/blob/master/src/tests/gssapi/t_credstore.c#L779
            gssapi_sys::gss_store_cred_into(
                &mut minor_status,
                self.cred_handle,
                input_usage,
                desired_mech,
                overwrite_cred,
                default_cred,
                &mut gss_cred_store,
                &mut elements_stored.get_handle(),
                &mut cred_usage_stored
            )
        };
                
        if major_status == gssapi_sys::GSS_S_COMPLETE {
            Ok(true)
        } else {
            Err(Error::new(major_status, minor_status, OID::empty()))
        }
    }
}

impl Drop for Credentials {
    fn drop(&mut self) {
        let mut minor_status = 0;

        let major_status = unsafe {
            gssapi_sys::gss_release_cred(
                &mut minor_status,
                &mut self.cred_handle)
        };

        if major_status != gssapi_sys::GSS_S_COMPLETE {
            panic!("{}", Error::new(major_status, minor_status, OID::empty()))
        }
    }
}

#[cfg(feature = "services4user")]
pub struct CredentialsBuilder {
    desired_name: Name,
    time_req: u32,
    desired_mechs: OIDSet,
    cred_usage: i32,
    impersonator: Option<Credentials>
}

#[cfg(not(feature = "services4user"))]
pub struct CredentialsBuilder {
    desired_name: Name,
    time_req: u32,
    desired_mechs: OIDSet,
    cred_usage: i32,
}

impl CredentialsBuilder {
    #[cfg(feature = "services4user")]
    pub fn new<T: Into<Name>>(desired_name: T) -> Self {
        CredentialsBuilder {
            desired_name: desired_name.into(),
            time_req: 0,
            desired_mechs: OIDSet::c_no_oid_set(),
            cred_usage: 0,
            impersonator: None
        }
    }

    #[cfg(not(feature = "services4user"))]
    pub fn new<T: Into<Name>>(desired_name: T) -> Self {
        CredentialsBuilder {
            desired_name: desired_name.into(),
            time_req: 0,
            desired_mechs: OIDSet::empty().unwrap(),
            cred_usage: 0,
        }
    }

    pub fn time_req(mut self, time_req: u32) -> Self {
        self.time_req = time_req;
        self
    }
    
    #[cfg(feature = "services4user")]
    pub fn impersonator(mut self, impersonator: Credentials) -> Self {
        self.impersonator = Some(impersonator);
        self
    }
    
    pub fn desired_mechs(mut self, desired_mechs: OIDSet) -> Self {
        self.desired_mechs = desired_mechs;
        self
    }

    #[cfg(feature = "services4user")]
    pub fn build(self) -> Result<Credentials> {
        let mut minor_status = 0;
        let mut output_cred_handle: gssapi_sys::gss_cred_id_t = ptr::null_mut();
        let actual_mechs = try!(OIDSet::empty());
        let mut time_rec = 0;
        
        let major_status = match self.impersonator {
            None => unsafe {
                gssapi_sys::gss_acquire_cred(
                    &mut minor_status,
                    self.desired_name.get_handle(),
                    self.time_req,
                    self.desired_mechs.get_handle(),
                    self.cred_usage as gssapi_sys::gss_cred_usage_t,
                    &mut output_cred_handle,
                    &mut actual_mechs.get_handle(),
                    &mut time_rec,
                )
            },
            Some(cred) => unsafe {
                gssapi_sys::gss_acquire_cred_impersonate_name(
                    &mut minor_status,
                    cred.get_handle(),
                    self.desired_name.get_handle(),
                    self.time_req,
                    self.desired_mechs.get_handle(),
                    gssapi_sys::GSS_C_INITIATE,                /* Only initiate is supported by Kerberos */
                    &mut output_cred_handle,
                    &mut actual_mechs.get_handle(),
                    &mut time_rec,
                )
            },
        };
        
        if major_status == gssapi_sys::GSS_S_COMPLETE {
            Ok(Credentials {
                cred_handle: output_cred_handle,
                mechs: actual_mechs,
                time_rec: time_rec,
            })
        } else {
            Err(Error::new(major_status, minor_status, OID::empty()))
        }
    }

    #[cfg(not(feature = "services4user"))]
    pub fn build(self) -> Result<Credentials> {
        let mut minor_status = 0;
        let mut output_cred_handle = ptr::null_mut();
        let actual_mechs = try!(OIDSet::empty());
        let mut time_rec = 0;
        
        let major_status = unsafe {
            gssapi_sys::gss_acquire_cred(
                &mut minor_status,
                self.desired_name.get_handle(),
                self.time_req,
                self.desired_mechs.get_handle(),
                self.cred_usage as gssapi_sys::gss_cred_usage_t,
                &mut output_cred_handle,
                &mut actual_mechs.get_handle(),
                &mut time_rec,
            )
        };
        
        if major_status == gssapi_sys::GSS_S_COMPLETE {
            Ok(Credentials {
                cred_handle: output_cred_handle,
                mechs: actual_mechs,
                time_rec: time_rec,
            })
        } else {
            Err(Error::new(major_status, minor_status, OID::empty()))
        }
    }
}
