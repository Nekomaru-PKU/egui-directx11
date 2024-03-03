pub const fn zeroed<T>() -> T { unsafe { std::mem::zeroed() }}

pub fn unwrap<T, R, F>(f: F) -> Result<T, R>
where
    F: FnOnce(Option<*mut Option<T>>) -> Result<(), R> {
    let mut ret = None;
    f(Some(&mut ret))?;
    Ok(ret.unwrap())
}
