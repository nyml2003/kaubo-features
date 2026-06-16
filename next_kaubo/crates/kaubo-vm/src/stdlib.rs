//! 标准库 — prelude 模块 (print, type_of, assert, math)
//!
//! 所有函数以 Rust NativeFn 方式实现，注册到 VM 中供 Kaubo 调用

/// 标准库函数签名: (args: &[i64]) -> Result<i64, String>
pub type NativeFn = fn(args: &[i64]) -> Result<i64, String>;

/// 注册所有标准库函数
pub fn register_all() -> Vec<(&'static str, NativeFn)> {
    vec![
        ("print", print_fn),
        ("type_of", type_of_fn),
        ("assert", assert_fn),
        ("sqrt", |a| Ok(((*a.get(0).unwrap_or(&0) as f64).sqrt()) as i64)),
        ("sin",  |a| Ok(((*a.get(0).unwrap_or(&0) as f64).sin()) as i64)),
        ("cos",  |a| Ok(((*a.get(0).unwrap_or(&0) as f64).cos()) as i64)),
        ("floor", |a| Ok(((*a.get(0).unwrap_or(&0) as f64).floor()) as i64)),
        ("ceil", |a| Ok(((*a.get(0).unwrap_or(&0) as f64).ceil()) as i64)),
    ]
}

/// print 函数 — 返回要打印的值 (由 VM 捕获输出)
fn print_fn(args: &[i64]) -> Result<i64, String> {
    // v2: print returns the value, VM captures it
    Ok(*args.first().unwrap_or(&0))
}

/// type_of 函数 — 返回类型标识
fn type_of_fn(args: &[i64]) -> Result<i64, String> {
    // v2: 返回类型 tag (0=Int64, 1=Float64, 2=String)
    Ok(0) // simplified: always Int64
}

/// assert 函数
fn assert_fn(args: &[i64]) -> Result<i64, String> {
    let cond = *args.first().unwrap_or(&0);
    if cond == 0 {
        Err(args.get(1).map(|s| format!("assertion failed: {}", s)).unwrap_or_else(|| "assertion failed".into()))
    } else {
        Ok(cond)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_print() {
        assert_eq!(register_all()[0].1(&[42]), Ok(42));
    }

    #[test]
    fn test_assert_pass() {
        assert_eq!(assert_fn(&[1, 0]), Ok(1));
    }

    #[test]
    fn test_assert_fail() {
        assert!(assert_fn(&[0, 0]).is_err());
    }

    #[test]
    fn test_sqrt() {
        let sqrt = register_all()[3].1;
        assert_eq!(sqrt(&[25]), Ok(5));
    }
}
