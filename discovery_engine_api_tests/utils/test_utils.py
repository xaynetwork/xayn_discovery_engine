from strgen import StringGenerator as sg
import random
import string
import datetime


def generate_random_letters(size_len):
    return sg("[\l]{{{size}}}".format(size=size_len)).render()


def generate_random_numbers(size):
    return sg("[\d]{{{size}}}".format(size=size)).render()


def generate_random_alphanumerical(size_len):
    return sg("[\w]{{{size}}}".format(size=size_len)).render()


def generate_random(regexp):
    return sg("{regexp}".format(regexp=regexp)).render()


def generate_invalid_id():
    symbols = "%$&"
    random_length = random.randint(5, 10)
    middle_index = random_length // 2
    result = ''.join(random.choice(string.ascii_letters + string.digits + symbols) for i in range(random_length))
    result = result[:middle_index] + random.choice(symbols) + result[middle_index + 1:]
    return result


def get_current_date_time():
    now = datetime.datetime.now().isoformat()
    return now


def get_updated_date_time(add_hours):
    now = datetime.datetime.now()
    updated_time = now + datetime.timedelta(hours=add_hours)
    return updated_time.isoformat()
