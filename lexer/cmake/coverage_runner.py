import os
import sys
import subprocess
import argparse
from pathlib import Path


def run_command(command, description, env=None):
    """执行外部命令并处理错误，支持自定义环境变量"""
    try:
        print(f"[Coverage] {description}...")
        # 确保命令中的路径是字符串格式
        command_str = [str(c) for c in command]
        result = subprocess.run(
            command_str,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )
        print(f"[Coverage] {description} 成功")
        return result.stdout
    except subprocess.CalledProcessError as e:
        print(f"[Coverage] {description} 失败（退出码：{e.returncode}）")
        print(f"[错误输出]\n{e.stderr}")
        sys.exit(e.returncode)
    except Exception as e:
        print(f"[Coverage] {description} 发生错误：{str(e)}")
        sys.exit(1)


def main():
    # 解析命令行参数
    parser = argparse.ArgumentParser(description="覆盖率全流程处理脚本（跨平台）")
    parser.add_argument("--test-exe", required=True, help="测试程序的完整路径")
    parser.add_argument(
        "--coverage-name", required=True, help="覆盖率数据文件前缀（无需扩展名）"
    )
    parser.add_argument(
        "--src-dir", required=True, help="源代码根目录（用于计算覆盖率）"
    )
    parser.add_argument("--output-dir", required=True, help="报告输出根目录")
    # 添加LLVM工具路径参数，支持自定义路径
    parser.add_argument(
        "--llvm-profdata", help="llvm-profdata工具的路径（默认从系统PATH查找）"
    )
    parser.add_argument("--llvm-cov", help="llvm-cov工具的路径（默认从系统PATH查找）")

    args = parser.parse_args()

    # 确定LLVM工具路径 - 优先使用参数提供的路径
    llvm_profdata = args.llvm_profdata or "llvm-profdata"
    llvm_cov = args.llvm_cov or "llvm-cov"

    # 路径处理（跨平台兼容）
    test_exe = Path(args.test_exe).resolve()
    src_dir = Path(args.src_dir).resolve()
    output_dir = Path(args.output_dir).resolve()
    profraw_path = output_dir / f"{args.coverage_name}.profraw"
    profdata_path = output_dir / f"{args.coverage_name}.profdata"
    text_report_dir = output_dir / "coverage_report"
    html_report_dir = output_dir / "coverage_html"

    # 验证输入
    if not test_exe.exists():
        print(f"[错误] 测试程序不存在：{test_exe}")
        sys.exit(1)
    if not src_dir.exists():
        print(f"[错误] 源代码目录不存在：{src_dir}")
        sys.exit(1)

    # 创建输出目录
    output_dir.mkdir(parents=True, exist_ok=True)
    text_report_dir.mkdir(parents=True, exist_ok=True)
    html_report_dir.mkdir(parents=True, exist_ok=True)

    # 1. 运行测试生成原始覆盖率数据（.profraw）
    env = os.environ.copy()
    env["LLVM_PROFILE_FILE"] = str(profraw_path)  # 跨平台环境变量设置

    run_command(
        [test_exe],  # 列表形式传递，避免shell解析问题
        f"运行测试程序生成原始数据（{profraw_path.name}）",
        env=env,  # 传递包含覆盖率输出路径的环境变量
    )

    # 检查profraw文件是否生成
    if not profraw_path.exists():
        print(f"[错误] 未生成原始覆盖率数据文件：{profraw_path}")
        print(f"[提示] 请检查测试程序是否正常运行并生成输出")
        sys.exit(1)

    # 2. 合并原始数据为.profdata
    run_command(
        [
            llvm_profdata,
            "merge",
            "-sparse",
            profraw_path,
            "-o",
            profdata_path,
        ],
        f"合并原始数据为 {profdata_path.name}",
    )

    # 3. 生成文本报告
    text_report_path = text_report_dir / f"{args.coverage_name}_report.txt"
    try:
        print(f"[Coverage] 生成文本报告...")
        result = subprocess.run(
            [
                llvm_cov,
                "report",
                test_exe,
                "-instr-profile",
                profdata_path,
                str(src_dir),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode != 0:
            print(f"[错误] 生成文本报告失败：{result.stderr}", file=sys.stderr)
            sys.exit(result.returncode)

        with open(text_report_path, "w", encoding="utf-8") as f:
            f.write(result.stdout)
        print(f"[Coverage] 文本报告生成成功：{text_report_path}")
    except Exception as e:
        print(f"[错误] 生成文本报告时发生异常：{str(e)}")
        sys.exit(1)

    # 4. 生成HTML报告
    run_command(
        [
            llvm_cov,
            "show",
            test_exe,
            "-instr-profile",
            profdata_path,
            "-format=html",
            f"-output-dir={html_report_dir}",
            "-show-line-counts-or-regions",
            "-show-instantiations=false",  # 不显示模板实例化
            str(src_dir),
        ],
        f"生成HTML报告（{html_report_dir}）",
    )

    print("\n[Coverage] 全流程完成！")
    print(f"[Coverage] 文本报告：{text_report_path}")
    print(f"[Coverage] HTML报告：{html_report_dir / 'index.html'}")


if __name__ == "__main__":
    main()
