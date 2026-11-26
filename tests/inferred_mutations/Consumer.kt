fun consume(): Boolean {
    val counter = InferredCounter(5)
    return counter.update(true) == 7 && counter.isStored() && counter.update(false) == 6 && !counter.isStored() && counter.setterCalls == 2
}
