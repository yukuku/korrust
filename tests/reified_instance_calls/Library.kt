package fixture
class Scope(val label: String) {
    inline fun <reified T> accepts(input: Any): Boolean = input is T
    inline fun <reified T> checked(input: Any, action: (String) -> T): T? {
        if (input !is T) return null
        return action(label)
    }
}
