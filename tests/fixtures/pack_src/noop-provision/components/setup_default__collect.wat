(module
  (memory (export "memory") 1)
  (data (i32.const 0) "{\"diagnostics\":[],\"plan\":{\"config_patch\":{},\"secrets_patch\":{\"set\":{},\"delete\":[]},\"webhook_ops\":[],\"subscription_ops\":[],\"oauth_ops\":[],\"notes\":[\"collect step\"]},\"questions\":{\"type\":\"AdaptiveCard\",\"version\":\"1.4\",\"body\":[{\"type\":\"TextBlock\",\"text\":\"Provide settings\"}]}}")
  (func (export "run") (param i32 i32) (result i32 i32)
    i32.const 0
    i32.const 271
  )
)
