# zCore Tutorial

[![CI](https://github.com/rcore-os/zCore-Tutorial/workflows/CI/badge.svg?branch=master)](https://github.com/rcore-os/zCore-Tutorial/actions)
[![Docs](https://img.shields.io/badge/docs-alpha-blue)](https://rcore-os.github.io/zCore-Tutorial/)

## 仓库目录

* `docs/`: 教学实验指导
* `zcore`: 操作系统代码

## 实验指导

基于 mdBook，目前目前已经部署到了 [GitHub Pages](https://rcore-os.github.io/zCore-Tutorial/) 上面。

### 文档本地使用方法

```bash
git clone https://github.com/rcore-os/zCore-Tutorial.git
cd zCore-Tutorial
cargo install mdbook
mdbook serve docs
```

## code
`code`目录下的`rust-toolchain`内容为`nightly-2021-07-27`。原则上，我们会采用`rustc`最新的版本。目前的版本信息如下：
```
rustc 1.56.0-nightly (08095fc1f 2021-07-26)
```
