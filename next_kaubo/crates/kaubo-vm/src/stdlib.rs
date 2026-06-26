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
        ("sqrt", |a| {
            let value = *a.first().ok_or("sqrt expects 1 argument")?;
            Ok((f64::from_bits(value as u64)).sqrt().to_bits() as i64)
        }),
        ("sin", |a| {
            let value = *a.first().ok_or("sin expects 1 argument")?;
            Ok((f64::from_bits(value as u64)).sin().to_bits() as i64)
        }),
        ("cos", |a| {
            let value = *a.first().ok_or("cos expects 1 argument")?;
            Ok((f64::from_bits(value as u64)).cos().to_bits() as i64)
        }),
        ("floor", |a| {
            let value = *a.first().ok_or("floor expects 1 argument")?;
            Ok((f64::from_bits(value as u64)).floor().to_bits() as i64)
        }),
        ("ceil", |a| {
            let value = *a.first().ok_or("ceil expects 1 argument")?;
            Ok((f64::from_bits(value as u64)).ceil().to_bits() as i64)
        }),
    ]
}

/// print 函数 — 返回要打印的值 (由 VM 捕获输出)
fn print_fn(args: &[i64]) -> Result<i64, String> {
    // v2: print returns the value, VM captures it
    args.first()
        .copied()
        .ok_or_else(|| "print expects 1 argument".into())
}

/// type_of 函数 — 返回类型标识
fn type_of_fn(_args: &[i64]) -> Result<i64, String> {
    Err("type_of is not implemented".into())
}

/// assert 函数
fn assert_fn(args: &[i64]) -> Result<i64, String> {
    let cond = *args.first().ok_or("assert expects at least 1 argument")?;
    if cond == 0 {
        Err(args
            .get(1)
            .map(|s| format!("assertion failed: {}", s))
            .unwrap_or_else(|| "assertion failed".into()))
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
        assert_eq!(
            sqrt(&[25.0f64.to_bits() as i64]),
            Ok(5.0f64.to_bits() as i64)
        );
    }

    #[test]
    fn register_all_exposes_expected_functions() {
        let names: Vec<_> = register_all().into_iter().map(|(name, _)| name).collect();
        assert_eq!(
            names,
            vec!["print", "type_of", "assert", "sqrt", "sin", "cos", "floor", "ceil"]
        );
    }

    #[test]
    fn type_of_is_stable_placeholder() {
        assert!(type_of_fn(&[]).is_err());
    }

    #[test]
    fn math_helpers_reject_missing_args() {
        for (_, func) in register_all().into_iter().skip(3) {
            assert!(func(&[]).is_err());
        }
    }

    #[test]
    fn test_sin() {
        let sin = register_all()[4].1;
        let result = sin(&[std::f64::consts::PI.to_bits() as i64]).unwrap();
        let val = f64::from_bits(result as u64);
        // sin(pi) should be close to 0
        assert!(val.abs() < 1e-10);
    }

    #[test]
    fn test_cos() {
        let cos = register_all()[5].1;
        let result = cos(&[0.0f64.to_bits() as i64]).unwrap();
        let val = f64::from_bits(result as u64);
        assert!((val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_floor() {
        let floor = register_all()[6].1;
        let result = floor(&[3.7f64.to_bits() as i64]).unwrap();
        let val = f64::from_bits(result as u64);
        assert_eq!(val, 3.0);
    }

    #[test]
    fn test_ceil() {
        let ceil = register_all()[7].1;
        let result = ceil(&[3.2f64.to_bits() as i64]).unwrap();
        let val = f64::from_bits(result as u64);
        assert_eq!(val, 4.0);
    }

    #[test]
    fn test_print_requires_one_arg() {
        assert!(print_fn(&[]).is_err());
    }

    #[test]
    fn test_sqrt_of_four() {
        let sqrt = register_all()[3].1;
        assert_eq!(
            sqrt(&[4.0f64.to_bits() as i64]).unwrap(),
            2.0f64.to_bits() as i64
        );
    }

    #[test]
    fn test_sqrt_of_zero() {
        let sqrt = register_all()[3].1;
        assert_eq!(
            sqrt(&[0.0f64.to_bits() as i64]).unwrap(),
            0.0f64.to_bits() as i64
        );
    }

    #[test]
    fn test_sin_of_zero() {
        let sin = register_all()[4].1;
        assert_eq!(sin(&[0]).unwrap(), 0);
    }

    #[test]
    fn test_cos_of_zero() {
        let cos = register_all()[5].1;
        let result = cos(&[0]).unwrap();
        let val = f64::from_bits(result as u64);
        assert!((val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_floor_of_int() {
        let floor = register_all()[6].1;
        let result = floor(&[5.0f64.to_bits() as i64]).unwrap();
        assert_eq!(f64::from_bits(result as u64), 5.0);
    }

    #[test]
    fn test_ceil_of_int() {
        let ceil = register_all()[7].1;
        let result = ceil(&[5.0f64.to_bits() as i64]).unwrap();
        assert_eq!(f64::from_bits(result as u64), 5.0);
    }

    #[test]
    fn test_assert_with_message() {
        let err = assert_fn(&[0, 42]).unwrap_err();
        assert!(err.contains("42"));
    }

    #[test]
    fn test_assert_truthy_returns_cond() {
        assert_eq!(assert_fn(&[42]), Ok(42));
        assert_eq!(assert_fn(&[1]), Ok(1));
        assert_eq!(assert_fn(&[-1]), Ok(-1));
    }
}
