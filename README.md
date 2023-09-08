# nihility-terminal

理想中是制作一个可以随意添加/卸载模块的助手系统。

## 功能列表：

- [ ] 中文拼音识别（离线识别，尽可能低消耗，难点！）
- [ ] 命令分类（核心）
- [ ] 模块热加载/卸载（包括本地模块、局域网模块、API模块）
- [ ] 跨平台GUI、web管理（跨平台GUI用于交互）

## Grpc消息设计

- [ ] 模块信息
- [ ] 指令信息
- [ ] 操作信息

## 配置项设计

### 日志模块（log）

- [ ] （enable）是否启用日志，默认为启用，（bool）
- [ ] （level）日志级别，默认为`INFO`，（string）
- [ ] （with_file）日志是否显示文件，默认为`false`，（bool）
- [ ] （with_line_number）日志是否显示行号，默认为`false`，（bool）
- [ ] （with_thread_ids）日志是否显示线程id，默认为`false`，（bool）
- [ ] （with_target）日志是否显示事件目标，默认为`false`，（bool）

### Grpc模块（grpc）

- [ ] （enable）是否启用Grpc通讯，Grpc和管道至少要启用一个，否则报错（bool）
- [ ] （addr）Grpc服务地址，即本机地址，默认由程序获取机器内网地址（string）
- [ ] （port）Grpc服务端口，即服务使用的端口，默认为5050（number）

### 管道通讯模块（pipe）

此模块在不同平台使用的方式不同，unix平台使用pipe，Windows平台使用named_pipe

- [ ] （enable）是否启用管道通讯，Grpc和管道至少要启用一个，否则报错，当没有配置文件时，默认使用管道通信（bool）

#### unix平台（unix）

- [ ] （directory）管道存放位置，只在程序目录下创建，默认为`communication`，注意校验！（string）
- [ ] （module）模块注册管道，默认为`module`，（string）
- [ ] （instruct_receiver）指令接收管道，默认为`instruct_receiver`，（string）
- [ ] （instruct_sender）指令发送管道，默认为`instruct_sender`，（string）
- [ ] （manipulate_receiver）操作接收管道，默认为`manipulate_receiver`，（string）
- [ ] （manipulate_sender）操作发送管道，默认为`manipulate_sender`，（string）

#### windows平台（windows）

- [ ] TODO

### 组播模块（multicast）

- [ ] （enable）是否启用组播，当Grpc开启时默认启用（bool）
- [ ] （bind_addr）udp绑定地址，默认为`0.0.0.0`，（string）
- [ ] （bind_port）udp绑定端口，默认为`0`，（number）
- [ ] （multicast_group）组播组，默认为`224.0.0.123`，（string）
- [ ] （multicast_port）组播端口，默认为`1234`，（number）
- [ ] （interval）组播发送间隔，默认为`5`，单位：秒，（number）

### 子模块管理模块（module_manager)

- [ ] （interval）处理消息的间隔，默认为`1`，单位：秒，（number）

#### 默认子模块（default_submodule）

- [ ] （enable）是否使用内置子模块，当没有配置文件时默认开启（bool）
- [ ] TODO
