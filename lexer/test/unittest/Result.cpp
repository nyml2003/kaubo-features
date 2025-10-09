
#include "Utils/Result.h"
#include <gtest/gtest.h>
#include <string>

using namespace utils;  // NOLINT

class ResultTest : public ::testing::Test {
 protected:
  void SetUp() override {
    // 在每个测试用例之前执行的操作
  }

  void TearDown() override {
    // 在每个测试用例之后执行的操作
  }

  // 通用的转换函数，支持多种类型
  static auto thenFunc(int x) noexcept { return x * 2; };
  static auto thenFuncForString(const std::string& x) noexcept {
    return x + "2";
  };

  // 通用的Result返回函数，支持多种类型组合
  template <typename T, typename E>
  static auto thenResultFunc(T x) noexcept -> Result<T, E> {
    if constexpr (std::is_same_v<T, int>) {
      return ok(x * 2);
    } else if constexpr (std::is_same_v<T, std::string>) {
      return ok(x + "2");
    }
  }

  template <typename T, typename E>
  static auto catchResultFunc(const E& x) noexcept -> Result<T, E> {
    if constexpr (std::is_same_v<E, int>) {
      return err(x * 2);
    } else if constexpr (std::is_same_v<E, std::string>) {
      return err(x + "2");
    }
  }

  // void类型支持
  static auto voidThenFunc() noexcept { return 42; }
  template <typename T, typename E>
  static auto voidThenResultFunc() noexcept -> Result<T, E> {
    if constexpr (std::is_same_v<T, int>) {
      return ok(42);
    } else if constexpr (std::is_same_v<T, std::string>) {
      return ok(std::string("42"));
    }
  }
  template <typename T, typename E>
  static auto voidCatchResultFunc() noexcept -> Result<T, E> {
    if constexpr (std::is_same_v<E, int>) {
      return err(42);
    } else if constexpr (std::is_same_v<E, std::string>) {
      return err(std::string("42"));
    }
  }
};

// 测试场景：T和E均为非void类型且类型不同，验证ok值的正确性
TEST_F(ResultTest, NonVoidTNonVoidEDifferentTypeOkValueCorrect) {
  Result<int, std::string> result = ok(42);
  EXPECT_TRUE(result.is_ok());
  EXPECT_FALSE(result.is_err());
  EXPECT_EQ(result.unwrap(), 42);

  EXPECT_EQ(result.map(thenFunc).unwrap(), 84);
  EXPECT_EQ(result.map_err(thenFuncForString).unwrap(), 42);
  EXPECT_EQ(result.and_then(thenResultFunc<int, std::string>).unwrap(), 84);
  EXPECT_EQ(result.or_else(catchResultFunc<int, std::string>).unwrap(), 42);
}

// 测试场景：T和E均为非void类型且类型不同，验证err值的正确性
TEST_F(ResultTest, NonVoidTNonVoidEDifferentTypeErrValueCorrect) {
  Result<int, std::string> result = err(std::string("error"));
  EXPECT_FALSE(result.is_ok());
  EXPECT_TRUE(result.is_err());
  EXPECT_EQ(result.unwrap_err(), std::string("error"));
  EXPECT_EQ(result.map(thenFunc).unwrap_err(), std::string("error"));
  EXPECT_EQ(
    result.map_err(thenFuncForString).unwrap_err(), std::string("error2")
  );
  EXPECT_EQ(
    result.and_then(thenResultFunc<int, std::string>).unwrap_err(),
    std::string("error")
  );
  EXPECT_EQ(
    result.or_else(catchResultFunc<int, std::string>).unwrap_err(),
    std::string("error2")
  );
}

// 测试场景：T和E均为非void类型且类型相同，验证ok值的正确性
TEST_F(ResultTest, NonVoidTNonVoidESameTypeOkValueCorrect) {
  Result<int, int> result = ok(42);  // T和E均为int（相同类型）
  EXPECT_TRUE(result.is_ok());
  EXPECT_FALSE(result.is_err());
  EXPECT_EQ(result.unwrap(), 42);
  EXPECT_EQ(result.map(thenFunc).unwrap(), 84);
  EXPECT_EQ(result.map_err(thenFunc).unwrap(), 42);
  EXPECT_EQ(result.and_then(thenResultFunc<int, int>).unwrap(), 84);
  EXPECT_EQ(result.or_else(catchResultFunc<int, int>).unwrap(), 42);
}

// 测试场景：T为非void、E为非void类型且类型相同，验证err值的正确性
TEST_F(ResultTest, NonVoidTNonVoidESameTypeErrValueCorrect) {
  Result<int, int> result = err(42);  // T和E均为int（相同类型）
  EXPECT_FALSE(result.is_ok());
  EXPECT_TRUE(result.is_err());
  EXPECT_EQ(result.unwrap_err(), 42);
  EXPECT_EQ(result.map(thenFunc).unwrap_err(), 42);
  EXPECT_EQ(result.map_err(thenFunc).unwrap_err(), 84);
  EXPECT_EQ(result.and_then(thenResultFunc<int, int>).unwrap_err(), 42);
  EXPECT_EQ(result.or_else(catchResultFunc<int, int>).unwrap_err(), 84);
}

// 测试场景：T为void、E为非void类型，验证ok状态的正确性
TEST_F(ResultTest, VoidTNonVoidEOkStateCorrect) {
  Result<void, std::string> result = ok();  // T为void时ok无参数
  EXPECT_TRUE(result.is_ok());
  EXPECT_FALSE(result.is_err());
  EXPECT_NO_THROW(result.unwrap());  // 验证void类型的unwrap不报错
  EXPECT_EQ(result.map(voidThenFunc).unwrap(), 42);
  EXPECT_NO_THROW(result.map_err(thenFuncForString).unwrap());
  EXPECT_EQ(result.and_then(voidThenResultFunc<int, std::string>).unwrap(), 42);
  EXPECT_NO_THROW(result.or_else(catchResultFunc<void, std::string>).unwrap());
}

// 测试场景：T为void、E为非void类型，验证err值的正确性
TEST_F(ResultTest, VoidTNonVoidEErrValueCorrect) {
  Result<void, std::string> result = err(std::string("error"));
  EXPECT_FALSE(result.is_ok());
  EXPECT_TRUE(result.is_err());
  EXPECT_EQ(result.unwrap_err(), std::string("error"));
  EXPECT_EQ(result.map(voidThenFunc).unwrap_err(), std::string("error"));
  EXPECT_EQ(
    result.map_err(thenFuncForString).unwrap_err(), std::string("error2")
  );
  EXPECT_EQ(
    result.and_then(voidThenResultFunc<void, std::string>).unwrap_err(),
    std::string("error")
  );
  EXPECT_EQ(
    result.or_else(catchResultFunc<void, std::string>).unwrap_err(),
    std::string("error2")
  );
}

// 测试场景：T为非void、E为void类型，验证ok值的正确性
TEST_F(ResultTest, NonVoidTVoidEOkValueCorrect) {
  Result<int, void> result = ok(42);  // E为void类型
  EXPECT_TRUE(result.is_ok());
  EXPECT_FALSE(result.is_err());
  EXPECT_EQ(result.unwrap(), 42);
  EXPECT_EQ(result.map(thenFunc).unwrap(), 84);
  EXPECT_EQ(result.map_err(voidThenFunc).unwrap(), 42);
  EXPECT_EQ(result.and_then(thenResultFunc<int, void>).unwrap(), 84);
  EXPECT_EQ(result.or_else(voidCatchResultFunc<int, std::string>).unwrap(), 42);
}

// 测试场景：T为非void、E为void类型，验证err状态的正确性
TEST_F(ResultTest, NonVoidTVoidEErrStateCorrect) {
  Result<int, void> result = err();  // E为void类型
  EXPECT_FALSE(result.is_ok());
  EXPECT_TRUE(result.is_err());
  EXPECT_NO_THROW(result.unwrap_err());  // 验证void类型的unwrap_err不报错
  EXPECT_NO_THROW(result.map(thenFunc).unwrap_err());
  EXPECT_EQ(result.map_err(voidThenFunc).unwrap_err(), 42);
  EXPECT_NO_THROW(result.and_then(thenResultFunc<int, void>).unwrap_err());
  EXPECT_EQ(
    result.or_else(voidCatchResultFunc<int, std::string>).unwrap_err(),
    std::string("42")
  );
}

// 测试场景：T和E均为void类型，验证ok状态的正确性
TEST_F(ResultTest, VoidTVoidEOkStateCorrect) {
  Result<void, void> result = ok();  // T和E均为void类型
  EXPECT_TRUE(result.is_ok());
  EXPECT_FALSE(result.is_err());
  EXPECT_NO_THROW(result.unwrap());  // 验证void类型的unwrap不报错
  EXPECT_EQ(result.map(voidThenFunc).unwrap(), 42);
  EXPECT_NO_THROW(result.map_err(voidThenFunc).unwrap());
  EXPECT_EQ(result.and_then(voidThenResultFunc<int, void>).unwrap(), 42);
  EXPECT_NO_THROW(result.or_else(voidCatchResultFunc<void, std::string>).unwrap());
}

// 测试场景：T和E均为void类型，验证err状态的正确性
TEST_F(ResultTest, VoidTVoidEErrStateCorrect) {
  Result<void, void> result = err();  // T和E均为void类型
  EXPECT_FALSE(result.is_ok());
  EXPECT_TRUE(result.is_err());
  EXPECT_NO_THROW(result.unwrap_err());  // 验证void类型的unwrap_err不报错
  EXPECT_NO_THROW(result.map(voidThenFunc).unwrap_err());
  EXPECT_EQ(result.map_err(voidThenFunc).unwrap_err(), 42);
  EXPECT_NO_THROW(result.and_then(voidThenResultFunc<void, void>).unwrap_err());
  EXPECT_EQ(
    result.or_else(voidCatchResultFunc<void, std::string>).unwrap_err(),
    std::string("42")
  );
}

TEST_F(ResultTest, NestedMap) {
  auto test = [](int x) noexcept -> Result<double, std::string> {
    if (x == 0) {
      return ok(3.14);
    }
    return err(std::string("not even"));
  };

  auto result = Result<int, std::string>(ok(0)).map(test);
  EXPECT_TRUE(result.is_ok());
  EXPECT_EQ(result.unwrap().unwrap(), 3.14);
}

TEST_F(ResultTest, NestedAndThen) {
  auto test = [](int x) noexcept -> Result<double, std::string> {
    if (x == 0) {
      return ok(3.14);
    }
    return err(std::string("not even"));
  };

  auto result = Result<int, std::string>(ok(0)).and_then(test);
  EXPECT_TRUE(result.is_ok());
  EXPECT_EQ(result.unwrap(), 3.14);
}
