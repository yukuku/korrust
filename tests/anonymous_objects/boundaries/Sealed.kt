sealed interface A { fun n(): Int }; fun bad(): A = object : A { override fun n(): Int = 1 }
