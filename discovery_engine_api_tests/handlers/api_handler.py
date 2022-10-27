import requests
import allure
from config.config import Config
import logging

LOGGER = logging.getLogger(__name__)


class ApiHandler:

    def __init__(self):
        conf = Config()
        self.interactions = str(conf.interactions.format(user_id=conf.user_id))
        self.ingestion = str(conf.ingestion)

    def post_documents(self, doc_dict):
        id_array = []
        for key in doc_dict:
            req = self.send_post_request(self.ingestion, doc_dict[key])
            req.raise_for_status()
            id_array.append(key)
        return id_array

    def interact_with_documents(self, interaction_type):
        return self.send_patch_request(self.interactions, interaction_type)

    @allure.step
    def send_get_request(self, url):
        LOGGER.info("sending GET to " + url)
        return requests.get(url, timeout=2)

    @allure.step
    def send_post_request(self, url, data):
        LOGGER.info("sending POST to " + url)
        return requests.post(url, data)

    @allure.step
    def send_patch_request(self, url, data):
        LOGGER.info("sending PATCH to " + url)
        return requests.patch(url=url, data=data)

    @allure.step
    def send_put_request(self, url, data):
        LOGGER.info("sending PUT to " + url)
        return requests.put(url, data)
