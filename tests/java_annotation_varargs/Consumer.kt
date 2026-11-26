package annotationfixture
@Mark class Empty
@Mark("a", "b", label = "ok") class Many
@Mark("first", *arrayOf("second", "third")) class Spread
@Mark(value = ["named"]) class Named
@Preset class Default
@Classes(String::class, StringBuilder::class) class Bounds
