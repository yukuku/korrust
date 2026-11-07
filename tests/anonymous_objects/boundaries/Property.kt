interface A { val n: Int }; fun bad(): A = object : A { override val n: Int = 2 }
