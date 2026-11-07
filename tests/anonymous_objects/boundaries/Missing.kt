interface A { fun n(): Int; fun m(): Int }; fun bad(): A = object : A { override fun n(): Int = 1 }
