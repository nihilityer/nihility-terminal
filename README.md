# nihility-terminal

理想中是制作一个可以随意添加/卸载模块的助手系统。

## 功能列表：

- [ ] 中文拼音识别（离线识别，尽可能低消耗，难点！）
- [ ] 命令分类（核心）
- [ ] 模块热加载/卸载（包括本地模块、局域网模块、API模块）
- [ ] 跨平台GUI、web管理（跨平台GUI用于交互）

## 开发中笔记

`ort`需要需要设置`ORT_DYLIB_PATH`为`onnruntime`的lib文件路径：

```
ORT_DYLIB_PATH=D:\software\onnxruntime-win-x64-1.16.0\lib\onnxruntime.dll
```

此外，还需设置

```
ORT_STRATEGY=system
```

