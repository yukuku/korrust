interface A { fun n(): Long }; fun bad(): A = object : A { override fun n() = 1 }
