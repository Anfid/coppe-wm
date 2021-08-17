(module
  (export "add" (func $add))
  (export "handle" (func $handle))

  (import "env" "move_window" (func $move_window (param $id i32) (param $x i32) (param $y i32)))

  (func $add (param $x i32) (param $y i32) (result i32)
    local.get $x
    local.get $y
    i32.add
  )

  (func $handle
    i32.const 0
    i32.const 100
    i32.const 200
    call $move_window
  )
)
