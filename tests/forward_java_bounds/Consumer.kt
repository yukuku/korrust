package fixture
fun text(controller: Screen<String>): String = controller.setup().get()
fun echoed(): String = Controller.echo<String, CharSequence>("ok", "x")
