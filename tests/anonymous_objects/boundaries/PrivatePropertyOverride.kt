interface A { val n: Int }
fun bad(): A = object : A { private override val n: Int = 1 }
