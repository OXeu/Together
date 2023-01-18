# Watch Together 共赏后端
## 接口定义
### POST /v1/login
第三方登录
### /v1/room
POST 创建房间  
PUT 修改房间  
GET `/v1/room/id` 加入房间  
DELETE 删除房间/10分钟无人加入自动删除

### SOCKET
同步进度、聊天
进度同步：
建立一个HashMap,创建房间创建一个key，加入房间时根据key加入，key对应两个HashMap，一个存配置，一个存进度