package fixture
class Holder {
    enum class Status {
        FIRST, LAST;
        companion object { fun values(i: Int): Int = i + 2 }
    }
}
class Ordinary { fun values(): Int = 19 }
