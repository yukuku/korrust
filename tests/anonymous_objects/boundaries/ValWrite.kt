interface A { fun n(): Int }; fun bad(): A { val state = 1; return object : A { override fun n(): Int { state = 2; return state } } }
