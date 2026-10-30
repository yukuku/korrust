package annotation17

@Rich
@BinaryMark
@SourceMark("discarded")
class DefaultUse

@Rich(
    number = 9223372036854775807L,
    ratio = 2.5f,
    measure = -0.0,
    letter = '\uFFFF',
    mode = Mode.FIRST,
    type = IntArray::class,
    numbers = [7L, -8L],
    flags = [],
    letters = ['x', '\u0000'],
    ratios = [3.5f],
    measures = [4.5, -0.0],
    names = ["explicit", "\uDFFF"],
    modes = [Mode.SECOND],
    types = [String::class, Long::class, Array<Token>::class],
    nested = Nested("outer", 91L, Mode.FIRST),
    children = [Nested("inner", -3L, Mode.SECOND)],
)
class ExplicitUse

@Pack
class EmptyPack

@Pack(Nested("one", 1L, Mode.FIRST), Nested("two", 2L, Mode.SECOND))
class PositionalPack

@Pack(entries = [Nested("named", 3L, Mode.FIRST)])
class NamedPack

@Pack(*arrayOf(Nested("spread", 4L, Mode.SECOND)))
class SpreadPack

class Placed(@param:Rich(number = 11L) val input: Int) {
    @field:Rich(number = 12L)
    @get:Rich(number = 13L)
    @set:Rich(number = 14L)
    @setparam:Rich(number = 15L)
    var value: Int = input

    @property:Rich(number = 16L)
    val property: Int = input

    @Rich(number = 17L)
    fun method(@Rich(number = 18L) argument: Int): Int = argument
}
