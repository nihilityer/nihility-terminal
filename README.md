# nihility-terminal

理想中是制作一个可以随意添加/卸载模块的助手系统。

## 功能列表：

- [x] 命令分类（核心）
- [x] 子模块热加载/卸载
- [ ] 多种模块间通讯方式
- [ ] 多种子模块管理方式
- [ ] 跨平台GUI、web管理（跨平台GUI用于交互）

## 对接注意事项

### Grpc子模块

子模块连接参数

- `grpc_addr`: 核心模块连接子模块的地址，示例: `http://127.0.0.1:1234`

### WindowsNamedPipe子模块

子模块连接参数

- `instruct_windows_named_pipe`: 核心模块向子模块发送指令的地址，示例: `\\.\pipe\nihilityer\submodule_name\instruct`
- `manipulate_windows_named_pipe`: 核心模块向子模块发送操作的地址，示例: `\\.\pipe\nihilityer\submodule_name\manipulate`

## 开发中笔记

`ort`需要需要设置`ORT_DYLIB_PATH`为`onnruntime`的lib文件路径：

```
ORT_DYLIB_PATH=D:\software\onnxruntime-win-x64-1.16.0\lib\onnxruntime.dll
```
