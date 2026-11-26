class Bad(private var value: Token?) { fun read(): Int { if (value != null) return value.value(); return 0 } }
