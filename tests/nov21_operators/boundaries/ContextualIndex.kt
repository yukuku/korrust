class Value { operator fun inc(): Value = this }
class Holder { operator fun get(index: Byte): Value = Value(); operator fun set(index: Byte, value: Value) {} }
fun use(value: Holder) { value[1]++ }
