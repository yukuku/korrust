package fixture
fun cast(value: Any?): Box<String> = value as Box<String>
fun safe(value: Any?): Box<String>? = value as? Box<String>
fun read(value: Any?): String = cast(value).value
fun test(value: Any?): Boolean = value is Box<*>
