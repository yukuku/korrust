operator fun UInt?.plus(other:UInt):UInt = (this ?: 0u).plus(other)
fun f(a:UInt?):UInt = a + 1u
