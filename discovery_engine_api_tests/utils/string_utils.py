from strgen import StringGenerator as sg


def generate_random_letters(size_len):
    return sg('[\l]{{{size}}}'.format(size=size_len)).render()


def generate_random_numbers(size):
    return sg('[\d]{{{size}}}'.format(size=size)).render()


def generate_random_alphanumerical(size_len):
    return sg('[\w]{{{size}}}'.format(size=size_len)).render()


def generate_random(regexp):
    return sg('{regexp}'.format(regexp=regexp)).render()

