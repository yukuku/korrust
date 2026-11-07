interface A<T> { fun n(value: T): T }; fun bad(): A<Int> = object : A<Int> { override fun n(value: Int): Int = value }
