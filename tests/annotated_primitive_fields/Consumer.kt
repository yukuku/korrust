package fixture
fun increment(value: Dimensions): Long {
    value.width = value.width + value.height
    Dimensions.count = Dimensions.count + value.width
    return Dimensions.count
}
