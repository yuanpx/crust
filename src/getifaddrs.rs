// Copyright 2015 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

use std::net::{IpAddr, Ipv4Addr};

/// Details about an interface on this host
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct IfAddr {
    /// The name of the interface
    pub name: String,
    /// The IP address of the interface
    pub addr: IpAddr,
    /// The netmask of the interface
    pub netmask: IpAddr,
    /// How to send a broadcast on the interface
    pub broadcast: IpAddr,
}

impl IfAddr {
    /// Create a new IfAddr
    pub fn new() -> IfAddr {
        IfAddr {
            name: String::new(),
            addr: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            netmask: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            broadcast: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
        }
    }
}

#[cfg(not(windows))]
mod getifaddrs_posix {
    use super::IfAddr;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::{mem, str};
    use std::ffi::CStr;
    use libc::consts::os::bsd44::{AF_INET, AF_INET6};
    use libc::funcs::bsd43::getifaddrs as posix_getifaddrs;
    use libc::funcs::bsd43::freeifaddrs as posix_freeifaddrs;
    use libc::types::os::common::bsd44::ifaddrs as posix_ifaddrs;
    use libc::types::os::common::bsd44::sockaddr as posix_sockaddr;
    use libc::types::os::common::bsd44::sockaddr_in as posix_sockaddr_in;
    use libc::types::os::common::bsd44::sockaddr_in6 as posix_sockaddr_in6;

    #[allow(unsafe_code)]
    fn sockaddr_to_ipaddr(sockaddr : *const posix_sockaddr) -> Option<IpAddr> {
        if sockaddr.is_null() { return None }
        if unsafe{*sockaddr}.sa_family as u32 == AF_INET as u32 {
            let ref sa = unsafe{*(sockaddr as *const posix_sockaddr_in)};
            Some(IpAddr::V4(Ipv4Addr::new(
                ((sa.sin_addr.s_addr>>0) & 255) as u8,
                ((sa.sin_addr.s_addr>>8) & 255) as u8,
                ((sa.sin_addr.s_addr>>16) & 255) as u8,
                ((sa.sin_addr.s_addr>>24) & 255) as u8,
            )))
        } else if unsafe{*sockaddr}.sa_family as u32 == AF_INET6 as u32 {
            let ref sa = unsafe{*(sockaddr as *const posix_sockaddr_in6)};
            // Ignore all fe80:: addresses as these are link locals
            if sa.sin6_addr.s6_addr[0]==0x80fe { None }
            Some(IpAddr::V6(Ipv6Addr::new(
                ((sa.sin6_addr.s6_addr[0] & 255)<<8) | ((sa.sin6_addr.s6_addr[0]>>8) & 255),
                ((sa.sin6_addr.s6_addr[1] & 255)<<8) | ((sa.sin6_addr.s6_addr[1]>>8) & 255),
                ((sa.sin6_addr.s6_addr[2] & 255)<<8) | ((sa.sin6_addr.s6_addr[2]>>8) & 255),
                ((sa.sin6_addr.s6_addr[3] & 255)<<8) | ((sa.sin6_addr.s6_addr[3]>>8) & 255),
                ((sa.sin6_addr.s6_addr[4] & 255)<<8) | ((sa.sin6_addr.s6_addr[4]>>8) & 255),
                ((sa.sin6_addr.s6_addr[5] & 255)<<8) | ((sa.sin6_addr.s6_addr[5]>>8) & 255),
                ((sa.sin6_addr.s6_addr[6] & 255)<<8) | ((sa.sin6_addr.s6_addr[6]>>8) & 255),
                ((sa.sin6_addr.s6_addr[7] & 255)<<8) | ((sa.sin6_addr.s6_addr[7]>>8) & 255),
            )))
        }
        else { None }
    }

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "nacl"))]
    fn do_broadcast(ifaddr : &posix_ifaddrs) -> IpAddr {
        match sockaddr_to_ipaddr(ifaddr.ifa_ifu) {
            Some(a) => a,
            None => IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        }
    }
    
    #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "ios"))]
    fn do_broadcast(ifaddr : &posix_ifaddrs) -> IpAddr {
        match sockaddr_to_ipaddr(ifaddr.ifa_dstaddr) {
            Some(a) => a,
            None => IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        }
    }
    
    /// Return a vector of IP details for all the valid interfaces on this host
    #[allow(unsafe_code)]
    pub fn getifaddrs() -> Vec<IfAddr> {
        let mut ret = Vec::<IfAddr>::new();
        let mut ifaddrs : *mut posix_ifaddrs;
        unsafe {
          ifaddrs = mem::uninitialized();
          if -1 == posix_getifaddrs(&mut ifaddrs) {
            panic!("failed to retrieve interface details from getifaddrs()");
          }
        }
            
        let mut _ifaddr = ifaddrs;
        let mut first = true;
        while !_ifaddr.is_null() {
            if first { first=false; }
            else { _ifaddr = unsafe { (*_ifaddr).ifa_next }; }
            if _ifaddr.is_null() { break; }
            let ref ifaddr = unsafe { *_ifaddr };
            // println!("ifaddr1={}, next={}", _ifaddr as u64, ifaddr.ifa_next as u64);
            if ifaddr.ifa_addr.is_null() {
                continue;
            }
            let mut item = IfAddr::new();
            let name = unsafe { CStr::from_ptr(ifaddr.ifa_name) }.to_bytes();
            item.name = item.name + str::from_utf8(name).unwrap();
            match sockaddr_to_ipaddr(ifaddr.ifa_addr) {
                Some(a) => item.addr = a,
                None => continue,
            };
            match sockaddr_to_ipaddr(ifaddr.ifa_netmask) {
                Some(a) => item.netmask = a,
                None => (),
            };
            if (ifaddr.ifa_flags & 2 /*IFF_BROADCAST*/) != 0 {
                item.broadcast = do_broadcast(ifaddr);
            }
            ret.push(item);
        }
        unsafe { posix_freeifaddrs(ifaddrs); }
        ret
    }
}
#[cfg(not(windows))]
pub fn getifaddrs() -> Vec<IfAddr> {
    getifaddrs_posix::getifaddrs()
}

#[cfg(windows)]
mod getifaddrs_windows {
    use super::IfAddr;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::{str, ptr};
    use std::ffi::CStr;
    use libc::types::common::c95::c_void;
    use libc::types::os::arch::c95::{c_char, c_ulong, size_t, c_int };
    use libc::types::os::arch::extra::*;          // libc source code says this is all the Windows integral types
    use libc::consts::os::extra::*;               // win32 status code, constants etc
    use libc::consts::os::bsd44::*;               // the winsock constants
    use libc::types::os::common::bsd44::*;        // the winsock types
    use libc;
    
    #[repr(C)]
    #[allow(bad_style)]
    struct SOCKET_ADDRESS {
        pub lpSockaddr : *const sockaddr,
        pub iSockaddrLength : c_int,
    }
    #[repr(C)]
    #[allow(bad_style)]
    struct IP_ADAPTER_UNICAST_ADDRESS {
        pub Length : c_ulong,
        pub Flags : DWORD,
        pub Next : *const IP_ADAPTER_UNICAST_ADDRESS,
        pub Address : SOCKET_ADDRESS,
        // Loads more follows, but I'm not bothering to map these for now
    }
    #[repr(C)]
    #[allow(bad_style)]
    struct IP_ADAPTER_ADDRESSES {
        pub Length : c_ulong,
        pub IfIndex : DWORD,
        pub Next : *const IP_ADAPTER_ADDRESSES,
        pub AdapterName : *const c_char,
        pub FirstUnicastAddress : *const IP_ADAPTER_UNICAST_ADDRESS,
        // Loads more follows, but I'm not bothering to map these for now
    }
    #[link(name="Iphlpapi")]
    extern "system" {
        pub fn GetAdaptersAddresses(family : c_ulong, flags : c_ulong, reserved : *const c_void, addresses : *const IP_ADAPTER_ADDRESSES, size : *mut c_ulong) -> c_ulong;
    }

    /// Return a vector of IP details for all the valid interfaces on this host
    #[allow(unsafe_code)]
    pub fn getifaddrs() -> Vec<IfAddr> {
        let mut ret = Vec::<IfAddr>::new();
        let mut ifaddrs : *const IP_ADAPTER_ADDRESSES;
        let mut buffersize : c_ulong = 15000;
        loop {
            unsafe {
                ifaddrs = libc::malloc(buffersize as size_t) as *mut IP_ADAPTER_ADDRESSES;
                if ifaddrs.is_null() {
                    panic!("Failed to allocate buffer in getifaddrs()");
                }
                let retcode = GetAdaptersAddresses(0,
                                                   0x3e /* GAA_FLAG_SKIP_ANYCAST|GAA_FLAG_SKIP_MULTICAST|GAA_FLAG_SKIP_DNS_SERVER|GAA_FLAG_INCLUDE_PREFIX|GAA_FLAG_SKIP_FRIENDLY_NAME */,
                                                   ptr::null(),
                                                   ifaddrs,
                                                   &mut buffersize) as c_int;
                match retcode {
                    ERROR_SUCCESS => break,
                    111 /*ERROR_BUFFER_OVERFLOW*/ => {
                        libc::free(ifaddrs as *mut c_void);
                        buffersize = buffersize * 2;
                        continue
                    },
                    _ => panic!("GetAdaptersAddresses() failed with error code {}", retcode)
                }
            }
        }
            
        let mut _ifaddr = ifaddrs;
        let mut first = true;
        while !_ifaddr.is_null() {
            if first { first=false; }
            else { _ifaddr = unsafe { (*_ifaddr).Next }; }
            if _ifaddr.is_null() { break; }
            let ref ifaddr = unsafe { &*_ifaddr };
            // println!("ifaddr1={}, next={}", _ifaddr as u64, ifaddr.ifa_next as u64);
            
            let mut addr = ifaddr.FirstUnicastAddress;
            if addr.is_null() { continue; }
            let mut firstaddr = true;
            while !addr.is_null() {
                if firstaddr { firstaddr=false; }
                else { addr = unsafe { (*addr).Next }; }
                if addr.is_null() { break; }

                let mut item = IfAddr::new();
                let name = unsafe { CStr::from_ptr(ifaddr.AdapterName) }.to_bytes();
                item.name = item.name + str::from_utf8(name).unwrap();

                let sockaddr = unsafe { (*addr).Address.lpSockaddr };
                if sockaddr.is_null() { continue; }
                if unsafe{*sockaddr}.sa_family as u32 == AF_INET as u32 {
                    let ref sa = unsafe{*(sockaddr as *const sockaddr_in)};
                    // Ignore all 169.254.x.x addresses as these are not active interfaces
                    if sa.sin_addr.s_addr & 65535 == 0xfea9 { continue; }
                    item.addr = IpAddr::V4(Ipv4Addr::new(
                        ((sa.sin_addr.s_addr>>0) & 255) as u8,
                        ((sa.sin_addr.s_addr>>8) & 255) as u8,
                        ((sa.sin_addr.s_addr>>16) & 255) as u8,
                        ((sa.sin_addr.s_addr>>24) & 255) as u8,
                    ));
                } else if unsafe{*sockaddr}.sa_family as u32 == AF_INET6 as u32 {
                    let ref sa = unsafe{*(sockaddr as *const sockaddr_in6)};
                    // Ignore all fe80:: addresses as these are link locals
                    if sa.sin6_addr.s6_addr[0]==0x80fe { continue; }
                    item.addr = IpAddr::V6(Ipv6Addr::new(
                        ((sa.sin6_addr.s6_addr[0] & 255)<<8) | ((sa.sin6_addr.s6_addr[0]>>8) & 255),
                        ((sa.sin6_addr.s6_addr[1] & 255)<<8) | ((sa.sin6_addr.s6_addr[1]>>8) & 255),
                        ((sa.sin6_addr.s6_addr[2] & 255)<<8) | ((sa.sin6_addr.s6_addr[2]>>8) & 255),
                        ((sa.sin6_addr.s6_addr[3] & 255)<<8) | ((sa.sin6_addr.s6_addr[3]>>8) & 255),
                        ((sa.sin6_addr.s6_addr[4] & 255)<<8) | ((sa.sin6_addr.s6_addr[4]>>8) & 255),
                        ((sa.sin6_addr.s6_addr[5] & 255)<<8) | ((sa.sin6_addr.s6_addr[5]>>8) & 255),
                        ((sa.sin6_addr.s6_addr[6] & 255)<<8) | ((sa.sin6_addr.s6_addr[6]>>8) & 255),
                        ((sa.sin6_addr.s6_addr[7] & 255)<<8) | ((sa.sin6_addr.s6_addr[7]>>8) & 255),
                    ));
                }
                else { continue; }
                ret.push(item);
            }
        }
        unsafe { libc::free(ifaddrs as *mut c_void); }
        ret
    }
}
#[cfg(windows)]
pub fn getifaddrs() -> Vec<IfAddr> {
    getifaddrs_windows::getifaddrs()
}

#[cfg(test)]
mod test {
    use super::getifaddrs;
    use std::net::IpAddr;
    
    #[test]
    fn test_getifaddrs() {
        let mut has_loopback4 = false;
        let mut has_loopback6 = false;
        for ifaddr in getifaddrs() {
            println!("   Interface {} has IP {} netmask {} broadcast {}", ifaddr.name,
                     ifaddr.addr, ifaddr.netmask, ifaddr.broadcast);
            match ifaddr.addr {
                IpAddr::V4(v4) => if v4.is_loopback() { has_loopback4=true; },
                IpAddr::V6(v6) => if v6.is_loopback() { has_loopback6=true; },
            }
        }
        // Quick sanity test, can't think of anything better
        assert_eq!(has_loopback4 || has_loopback6, true);
    }
}
