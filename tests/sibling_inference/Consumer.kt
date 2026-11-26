package fixture
fun test(): Int = count(one("a"), emptyList()) + count(emptyList(), one(3))
