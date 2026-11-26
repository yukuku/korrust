package fixture
object Container {
    interface Session { fun id(): Int }
    class Deep { interface More { fun next(): String } }
    private interface Hidden
}
