(module
  (memory (export "memory") 1)
  (data (i32.const 0) "{\"diagnostics\":[],\"plan\":{\"config_patch\":{\"foo\":\"bar\"},\"secrets_patch\":{\"set\":{\"token\":{\"redacted\":true,\"value\":null}},\"delete\":[]},\"webhook_ops\":[],\"subscription_ops\":[],\"oauth_ops\":[],\"notes\":[\"apply done\"]},\"questions\":null}")
  (func (export "run") (param i32 i32) (result i32 i32)
    i32.const 0
    i32.const 227
  )
)
