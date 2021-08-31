(module
  (import "env" "move_window" (func $move_window (param $id i32) (param $x i32) (param $y i32)))

  (func $handle
    i32.const 0
    i32.const 100
    i32.const 200
    call $move_window
  )

  (memory (;0;) 1)
  (global (;0;) i32 (i32.const 512))
  (data (;0;) (i32.const 512) "wat_minimal\00")

  (export "memory" (memory 0))
  (export "id" (global 0))
  (export "handle" (func $handle))
)
