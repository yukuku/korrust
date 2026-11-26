interface A { val n: Int }
fun bad(): A = object : A { val n: Int = 1 }
