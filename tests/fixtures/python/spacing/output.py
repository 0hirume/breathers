@decorate
def example():
    first = 1
    second = 2  # trailing

    # attached
    if first < second:
        work()
    else:
        other()

    save()

    return first
