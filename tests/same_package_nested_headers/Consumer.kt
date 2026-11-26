package fixture
class Consumer : Container.Session, Container.Deep.More {
    override fun id(): Int = 42
    override fun next(): String = "ready"
}
