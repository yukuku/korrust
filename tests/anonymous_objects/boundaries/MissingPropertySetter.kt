interface A { var n: Int }
fun bad(): A = object : A { override val n: Int = 1 }
