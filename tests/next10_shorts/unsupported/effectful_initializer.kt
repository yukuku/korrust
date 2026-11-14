fun initializer(): (Int) -> Short = { it.toShort() }; fun make() = ShortArray(2, initializer())
