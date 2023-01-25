import configparser
import logging
import os


class Config:
    """
    Class that fills out config.ini with urls
    """
    config_path = os.path.abspath(os.curdir) + "/config.ini"
    config = configparser.ConfigParser()
    ENDPOINTS_SECTION = "Endpoints"

    def __init__(self):
        self.config.read(self.config_path)
        try:
            self.config.add_section(self.ENDPOINTS_SECTION)
        except configparser.DuplicateSectionError:
            pass
        try:
            self.config.set(self.ENDPOINTS_SECTION, "INGESTION_URI", os.environ["INGESTION_URI"])
            self.config.set(self.ENDPOINTS_SECTION, "PERSONALIZATION_URI", os.environ["PERSONALIZATION_URI"])
        except KeyError as error:
            logging.error(error)
        with open(os.path.abspath(os.curdir) + "/config.ini", "w") as configfile:
            self.config.write(configfile)

    def get_ingestion_endpoint(self):
        self.config.read(self.config_path)
        return self.config.get(self.ENDPOINTS_SECTION, "INGESTION_URI") + "/documents"

    def get_personalization_endpoint(self):
        self.config.read(self.config_path)
        return self.config.get(self.ENDPOINTS_SECTION,
                               "PERSONALIZATION_URI") + "/users/{user_id}/personalized_documents"

    def get_interactions_endpoint(self):
        self.config.read(self.config_path)
        return self.config.get(self.ENDPOINTS_SECTION, "PERSONALIZATION_URI") + "/users/{user_id}/interactions"
