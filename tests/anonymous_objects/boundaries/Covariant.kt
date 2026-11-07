interface A { fun n(): Any }; fun bad(): A = object : A { override fun n(): String = "valid" }
