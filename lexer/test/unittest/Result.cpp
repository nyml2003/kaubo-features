#include "Result.h"  // 假设你的Result类定义在这个头文件中
#include <gtest/gtest.h>
#include <string>

using Result::Err;
using Result::Ok;

// 测试基本的Ok和Err创建及状态检查
TEST(ResultTest, BasicCreationAndStateCheck) {
  // 测试非void类型的Ok
  Result::Result<int, std::string> ok_result = Ok<int>(42);
  EXPECT_TRUE(ok_result.is_ok());
  EXPECT_FALSE(ok_result.is_err());

  // 测试非void类型的Err
  Result::Result<int, std::string> err_result =
    Err(std::string("error message"));
  EXPECT_FALSE(err_result.is_ok());
  EXPECT_TRUE(err_result.is_err());

  // 测试void类型的Ok
  Result::Result<void, int> void_ok = Ok();
  EXPECT_TRUE(void_ok.is_ok());
  EXPECT_FALSE(void_ok.is_err());

  // 测试void类型的Err
  Result::Result<void, int> void_err = Err(123);
  EXPECT_FALSE(void_err.is_ok());
  EXPECT_TRUE(void_err.is_err());
}

// 测试unwrap方法
TEST(ResultTest, UnwrapMethod) {
  // 测试正常unwrap Ok值
  Result::Result<int, std::string> ok_result = Ok(42);
  EXPECT_EQ(ok_result.unwrap(), 42);

  // 测试unwrap Err时抛出异常
  Result::Result<int, std::string> err_result = Err(std::string("error"));
  EXPECT_THROW(err_result.unwrap(), std::runtime_error);

  // 测试void类型的unwrap
  Result::Result<void, int> void_ok = Ok();
  EXPECT_NO_THROW(void_ok.unwrap());

  Result::Result<void, int> void_err = Err(123);
  EXPECT_THROW(void_err.unwrap(), std::runtime_error);
}

// 测试unwrap_err方法
TEST(ResultTest, UnwrapErrMethod) {
  // 测试unwrap_err获取错误值
  Result::Result<int, std::string> err_result = Err(std::string("test error"));
  EXPECT_EQ(err_result.unwrap_err(), "test error");

  // 测试在Ok上调用unwrap_err抛出异常
  Result::Result<int, std::string> ok_result = Ok(42);
  EXPECT_THROW(ok_result.unwrap_err(), std::runtime_error);

  // 测试void类型的unwrap_err
  Result::Result<void, std::string> void_err = Err(std::string("void error"));
  EXPECT_EQ(void_err.unwrap_err(), "void error");
}

// 测试expect方法
TEST(ResultTest, ExpectMethod) {
  // 测试正常expect Ok值
  Result::Result<int, std::string> ok_result = Ok(42);
  EXPECT_EQ(ok_result.expect("should not fail"), 42);

  // 测试expect Err时抛出指定消息
  Result::Result<int, std::string> err_result = Err(std::string("error"));
  try {
    err_result.expect("expected failure");
    FAIL() << "Expected std::runtime_error";
  } catch (const std::runtime_error& e) {
    EXPECT_STREQ(e.what(), "expected failure");
  }

  // 测试void类型的expect
  Result::Result<void, int> void_ok = Ok();
  EXPECT_NO_THROW(void_ok.expect("void ok should not fail"));

  Result::Result<void, int> void_err = Err(123);
  try {
    void_err.expect("void error expected");
    FAIL() << "Expected std::runtime_error";
  } catch (const std::runtime_error& e) {
    EXPECT_STREQ(e.what(), "void error expected");
  }
}

// 测试map方法
TEST(ResultTest, MapMethod) {
  // 测试Ok值的map转换
  Result::Result<int, std::string> ok_result = Ok(42);
  auto mapped = ok_result.map([](int x) { return x * 2; });
  EXPECT_TRUE(mapped.is_ok());
  EXPECT_EQ(mapped.unwrap(), 84);

  // 测试Err值的map不转换
  Result::Result<int, std::string> err_result = Err(std::string("error"));
  auto err_mapped = err_result.map([](int x) { return x * 2; });
  EXPECT_TRUE(err_mapped.is_err());
  EXPECT_EQ(err_mapped.unwrap_err(), "error");
}

// 测试flatten方法
TEST(ResultTest, FlattenMethod) {
  // 测试嵌套Ok的flatten
  Result::Result<int, std::string> inner_ok = Ok(42);
  Result::Result<Result::Result<int, std::string>, std::string> nested_ok =
    Ok(std::move(inner_ok));
  auto flattened = nested_ok.flatten();
  EXPECT_TRUE(flattened.is_ok());
  EXPECT_EQ(flattened.unwrap(), 42);

  // 测试外层Err的flatten
  Result::Result<Result::Result<int, std::string>, std::string> outer_err =
    Err(std::string("outer error"));
  auto outer_err_flattened = outer_err.flatten();
  EXPECT_TRUE(outer_err_flattened.is_err());
  EXPECT_EQ(outer_err_flattened.unwrap_err(), "outer error");

  // 测试内层Err的flatten
  Result::Result<int, std::string> inner = Err(std::string("inner error"));
  Result::Result<Result::Result<int, std::string>, std::string> err =
    Ok(std::move(inner));
  auto inner_err_flattened = err.flatten();
  EXPECT_TRUE(inner_err_flattened.is_err());
  EXPECT_EQ(inner_err_flattened.unwrap_err(), "inner error");
}

// 测试and_then方法
TEST(ResultTest, AndThenMethod) {
  // 测试正常的and_then链
  Result::Result<int, std::string> ok_result = Ok(42);
  auto and_then_result =
    ok_result.and_then([](int x) -> Result::Result<int, std::string> {
      return Ok(x * 2);
    });
  EXPECT_TRUE(and_then_result.is_ok());
  EXPECT_EQ(and_then_result.unwrap(), 84);

  // 测试Err的and_then不执行
  Result::Result<int, std::string> err_result = Err(std::string("error"));
  auto err_and_then =
    err_result.and_then([](int x) -> Result::Result<int, std::string> {
      return Ok(x * 2);
    });
  EXPECT_TRUE(err_and_then.is_err());
  EXPECT_EQ(err_and_then.unwrap_err(), "error");

  // 测试and_then返回Err
  Result::Result<int, std::string> ok_to_err = Ok(42);
  auto and_then_to_err =
    ok_to_err.and_then([](int) -> Result::Result<int, std::string> {
      return Err(std::string("converted to error"));
    });
  EXPECT_TRUE(and_then_to_err.is_err());
  EXPECT_EQ(and_then_to_err.unwrap_err(), "converted to error");
}

// 测试不同错误类型
TEST(ResultTest, DifferentErrorTypes) {
  Result::Result<int, int> int_err = Err(42);
  EXPECT_EQ(int_err.unwrap_err(), 42);

  Result::Result<std::string, float> float_err = Err(3.14F);
  EXPECT_EQ(float_err.unwrap_err(), 3.14F);

  struct CustomError {
    std::string msg;
    int code;
  };

  Result::Result<bool, CustomError> custom_err =
    Err(CustomError{.msg="custom", .code=500});
  EXPECT_EQ(custom_err.unwrap_err().msg, "custom");
  EXPECT_EQ(custom_err.unwrap_err().code, 500);
}