fun pack(vararg value: Long): LongArray = value; fun bad(value: IntArray): LongArray = pack(*value)
