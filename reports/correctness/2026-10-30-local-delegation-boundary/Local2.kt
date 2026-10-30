fun f(): Int { var value by kotlin.properties.Delegates.notNull<Int>(); value = 1; return value }
