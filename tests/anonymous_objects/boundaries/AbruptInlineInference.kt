interface A { fun n(): Int }; fun bad(): A = object : A { override fun n(): Int { run { return 7 }; return 9 } }
