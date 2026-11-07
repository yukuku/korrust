interface A { fun n(): Int }; fun bad(): A { var state: Int; return object : A { override fun n(): Int = state } }
