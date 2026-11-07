package anonymouslibrary
interface Parent { fun value(): Int }
interface Pair : Parent { fun shift(delta: Int): Int; fun label(): String = "pair" }
