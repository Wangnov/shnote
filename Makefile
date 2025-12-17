# shnote Makefile
# 常用开发命令

.PHONY: all build build-release check test test-verbose lint fmt clean install uninstall cov cov-html cov-open doc doc-open help
.PHONY: release-patch release-minor release-major release-dry

# 默认目标
all: check test

# ============ 构建 ============

# Debug 构建
build:
	cargo build

# Release 构建
build-release:
	cargo build --release

# 检查编译（不生成二进制）
check:
	cargo check

# ============ 测试 ============

# 运行所有测试
test:
	cargo test

# 详细输出测试
test-verbose:
	cargo test -- --nocapture

# 运行特定测试（用法：make test-one TEST=test_name）
test-one:
	cargo test $(TEST) -- --nocapture

# 串行运行测试（调试并发问题时使用）
test-serial:
	cargo test -- --test-threads=1

# ============ 代码质量 ============

# Clippy 检查（警告视为错误）
lint:
	cargo clippy -- -D warnings

# 格式化代码
fmt:
	cargo fmt

# 检查格式（不修改）
fmt-check:
	cargo fmt -- --check

# 完整检查（lint + fmt + test）
ci: fmt-check lint test

# ============ 覆盖率 ============

# 安装覆盖率工具（首次使用前运行）
cov-install:
	cargo install cargo-llvm-cov

# 生成覆盖率报告（终端输出）
cov:
	cargo llvm-cov --all-features

# 生成 HTML 覆盖率报告
cov-html:
	cargo llvm-cov --all-features --html

# 生成并打开 HTML 覆盖率报告
cov-open:
	cargo llvm-cov --all-features --open

# 生成 LCOV 格式报告（用于 CI）
cov-lcov:
	cargo llvm-cov --all-features --lcov --output-path lcov.info

# ============ 文档 ============

# 生成文档
doc:
	cargo doc --no-deps

# 生成并打开文档
doc-open:
	cargo doc --no-deps --open

# ============ 安装/卸载 ============

# 安装到系统
install:
	cargo install --path .

# 从系统卸载
uninstall:
	cargo uninstall shnote

# ============ 清理 ============

# 清理构建产物
clean:
	cargo clean

# 清理覆盖率数据
clean-cov:
	cargo llvm-cov clean --workspace

# ============ 发布 ============

# 预览发布（dry-run）
release-dry:
	cargo release patch

# 发布 patch 版本 (0.1.5 → 0.1.6)
release-patch:
	cargo release patch --execute

# 发布 minor 版本 (0.1.5 → 0.2.0)
release-minor:
	cargo release minor --execute

# 发布 major 版本 (0.1.5 → 1.0.0)
release-major:
	cargo release major --execute

# 检查发布准备情况
publish-check:
	cargo publish --dry-run

# ============ 帮助 ============

help:
	@echo "shnote 开发命令"
	@echo ""
	@echo "构建："
	@echo "  make build         - Debug 构建"
	@echo "  make build-release - Release 构建"
	@echo "  make check         - 检查编译"
	@echo ""
	@echo "测试："
	@echo "  make test          - 运行所有测试"
	@echo "  make test-verbose  - 详细输出测试"
	@echo "  make test-one TEST=name - 运行特定测试"
	@echo "  make test-serial   - 串行运行测试"
	@echo ""
	@echo "代码质量："
	@echo "  make lint          - Clippy 检查"
	@echo "  make fmt           - 格式化代码"
	@echo "  make fmt-check     - 检查格式"
	@echo "  make ci            - 完整 CI 检查"
	@echo ""
	@echo "覆盖率："
	@echo "  make cov-install   - 安装覆盖率工具"
	@echo "  make cov           - 终端覆盖率报告"
	@echo "  make cov-html      - HTML 覆盖率报告"
	@echo "  make cov-open      - 生成并打开 HTML 报告"
	@echo ""
	@echo "文档："
	@echo "  make doc           - 生成文档"
	@echo "  make doc-open      - 生成并打开文档"
	@echo ""
	@echo "发布："
	@echo "  make release-dry   - 预览发布（dry-run）"
	@echo "  make release-patch - 发布 patch 版本"
	@echo "  make release-minor - 发布 minor 版本"
	@echo "  make release-major - 发布 major 版本"
	@echo ""
	@echo "其他："
	@echo "  make install       - 安装到系统"
	@echo "  make uninstall     - 从系统卸载"
	@echo "  make clean         - 清理构建产物"
