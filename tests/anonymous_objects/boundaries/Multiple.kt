interface A { fun n(): Int }; interface B { fun b(): Int }; fun bad(): A = object : A, B { override fun n(): Int = 1; override fun b(): Int = 2 }
