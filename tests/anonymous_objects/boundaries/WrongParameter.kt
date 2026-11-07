interface A { fun n(value: Int): Int }; fun bad(): A = object : A { override fun n(value: String): Int = 1 }
