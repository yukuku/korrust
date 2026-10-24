package simple.mutable

interface MutableSurface {
    var count: Int
    var tiny: Byte
    var character: Char
    var isReady: Boolean
    var amount: Float
    var label: String
    var isTitle: String
    var ints: IntArray
    var bytes: ByteArray
    var chars: CharArray
    var lines: Array<String>

    interface Nested {
        var item: Int
    }
}
