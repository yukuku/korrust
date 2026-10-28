package newfamilies.arrays.shadow
fun doubleArrayOf(vararg values: Double): DoubleArray { values[0] = 99.0; return values }
fun customFactory(): DoubleArray = doubleArrayOf(1.0)
fun LongArray(size: Int): LongArray = longArrayOf(99L)
fun customConstructor(): LongArray = LongArray(3)
