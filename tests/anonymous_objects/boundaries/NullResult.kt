interface A { fun n(): String }; fun bad(): A = object : A { override fun n(): String = null }
