interface A { fun n(): Int }; fun bad(): A = object : A { override fun n(): Int = 1; fun extra(): Int = 2 }
