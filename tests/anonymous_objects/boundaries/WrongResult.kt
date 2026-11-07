interface A { fun n(): Int }; fun bad(): A = object : A { override fun n(): String = "bad" }
