class Bad(private val initial: Token?) { val value: Token? get() = initial; fun read(): Int { if (value != null) return value.value(); return 0 } }
