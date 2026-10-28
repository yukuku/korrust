fun init(): (Int) -> Double = { it.toDouble() }; fun bad() = DoubleArray(2, init())
