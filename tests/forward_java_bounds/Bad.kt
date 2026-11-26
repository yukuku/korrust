package fixture
fun bad(): String = Controller.echo<String, Number>("ok", 1)
