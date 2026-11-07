interface A { fun <T> n(value: T): T }; fun bad(): A = object : A { override fun <T> n(value: T): T = value }
