package fixture
fun bad(value: Any): String = (value as Box<Int>).value
