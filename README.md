# nihility-terminal

理想中是制作一个可以随意添加/卸载模块的助手系统。

## 功能列表：

- [x] 命令分类（核心）
- [x] 子模块热加载/卸载
- [ ] 多种模块间通讯方式
- [ ] 多种子模块管理方式
- [ ] 跨平台GUI、web管理（跨平台GUI用于交互）

## 开发中笔记

`ort`需要需要设置`ORT_DYLIB_PATH`为`onnruntime`的lib文件路径：

```
ORT_DYLIB_PATH=D:\software\onnxruntime-win-x64-1.16.0\lib\onnxruntime.dll
```
