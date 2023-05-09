import json


class ModelBase:
    def to_json(self):
        """
        Method used to serialize model objects to JSON looking structure
        MUST BE EXTENDED WITH ALL MODEL CLASSES
        :return: string with JSON structure
        """
        return json.dumps(self, default=lambda o: o.__dict__, sort_keys=False, indent=4)
